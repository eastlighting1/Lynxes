# Lynxes

Graph-native analytics engine built on Apache Arrow.

```python
import lynxes as lx

graph = lx.read_gf("graph.gf")
result = graph.lazy().expand(hops=2).collect()
```

See the [project documentation](https://github.com/lynxes) for full API reference.
