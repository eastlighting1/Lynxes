# Output Views

This page documents the rendering vocabulary used by `lynxes query --view ...`. The view choice changes only how the query result is rendered in the terminal. It does not change the underlying graph result itself.

## View Summary

| View | Purpose | Common related options |
| :--- | :--- | :--- |
| `summary` | High-level graph summary | usually none |
| `table` | Row-oriented table preview | `--rows`, `--sort-by`, `--attrs`, `--expand-attrs`, `--width`, `--ascii` |
| `head` | Front-of-result preview | `--rows`, `--sort-by`, `--attrs`, `--expand-attrs`, `--width`, `--ascii` |
| `tail` | End-of-result preview | `--rows`, `--sort-by`, `--attrs`, `--expand-attrs`, `--width`, `--ascii` |
| `info` | Structural summary | usually none |
| `describe` | Structured textual description | `--describe-mode` |
| `schema` | Schema-focused rendering | usually none |
| `glimpse` | Compact preview | `--rows`, `--sort-by`, `--attrs`, `--expand-attrs`, `--width`, `--ascii` |

## Notes

`--rows` matters only for preview-oriented views. `--describe-mode` matters only for `describe`. Options such as `--attrs`, `--expand-attrs`, `--width`, and `--ascii` are renderer controls rather than traversal controls.
