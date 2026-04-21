# Python Error Mapping

Lynxes is implemented in Rust, but the public Python surface should still feel like Python. That means low-level engine failures are translated into Python exception classes before they reach user code. This page is the quick map from the common Python exception types back to the kinds of graph failures that usually produced them.

## Mapping Summary

| Python exception | Typical Lynxes meaning |
| :--- | :--- |
| `KeyError` | Missing node, edge, column, or pattern alias |
| `TypeError` | Invalid Python argument type or incompatible value shape |
| `ValueError` | Invalid graph semantics, invalid direction, parse failure, schema failure, or unsupported configuration value |
| `NotImplementedError` | Present surface area that is not supported yet |
| `OSError` | File I/O failure |
| `RuntimeError` | Connector or remote execution failure |

## `KeyError`

In the Python binding, `KeyError` is used when the name or id you asked for does not exist in the current graph context. Typical causes include calling `neighbors(...)` with a missing node id, asking for a column that does not exist, or referring to a pattern alias that is not defined.

## `TypeError`

`TypeError` usually means the binding rejected the Python value before the engine could do meaningful graph work with it. Typical causes include passing a non-string where a node id or column name is expected, passing a non-path-like object to a read or write function, or passing the wrong wrapper type into a method that expects `Expr`, `AggExpr`, `NodeFrame`, or `EdgeFrame`.

## `ValueError`

`ValueError` covers a broad set of graph and format failures. Typical causes include invalid direction strings, malformed `.gf` input, duplicate node ids, dangling edges, missing reserved graph columns, schema mismatches, validation failures, and unsupported configuration values.

## `NotImplementedError`

`NotImplementedError` is used when the public name exists but the feature is not supported yet. This is different from a bad input. It means the call path is known, but Lynxes is explicitly not ready to honor it as a working feature.

## `OSError`

`OSError` is the file-system side of the boundary. It is raised when a path cannot be opened, created, or written successfully.

## `RuntimeError`

`RuntimeError` is most common with connectors and other integration paths. When a remote system fails, a credential is rejected, an endpoint cannot be reached, or connector execution breaks at collection time, Python generally sees that as `RuntimeError`.

## Notes

The exact low-level Rust error still matters for engine development, but Python callers should rely on the Python exception type first. In day-to-day use, that is the more stable and more useful contract.
