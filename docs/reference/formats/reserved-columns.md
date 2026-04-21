# Reserved Graph Columns

These columns carry graph semantics in Lynxes. They are not ordinary user attributes even when they live beside user-defined fields in the same Arrow batch or parquet file. If one of these columns is missing, malformed, or semantically inconsistent, Lynxes treats that as a graph-shape failure rather than as a harmless missing attribute.

## Node-Side Reserved Columns

| Column | Meaning | Required |
| :--- | :--- | :--- |
| `_id` | Unique node identifier. | Yes |
| `_label` | Node label or labels, depending on the source representation. | Yes |

## Edge-Side Reserved Columns

| Column | Meaning | Required |
| :--- | :--- | :--- |
| `_src` | Source node id for the edge. | Yes |
| `_dst` | Destination node id for the edge. | Yes |
| `_type` | Edge type identifier. | Yes |
| `_direction` | Edge direction semantics. | Yes |

## Notes

These names matter because they are the bridge between ordinary columnar storage and graph meaning. `_id` is not just another string column; it defines node identity. `_src` and `_dst` are not just two attributes; they define graph connectivity. That is why loader failures involving these fields usually show up as validation errors instead of quiet coercions.
