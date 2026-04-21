# `.gf` Format

`.gf` is the human-readable text format used by Lynxes for authored graph files. It is the right format when the graph should be readable and editable by a person, and it is especially useful for examples, fixtures, and small reproducible test cases.

This page stays close to the accepted shape of the format. If you are actively authoring a new file and want the larger writing flow, schema examples, metadata examples, literal guidance, and style recommendations in one place, use the companion [`.gf` authoring guide](../../gf_authoring_guide.md).

## Smallest Useful Example

```gf
(alice: Person { age: 30 })
(bob: Person { age: 22 })

alice -[KNOWS]-> bob
```

## Node Form

Nodes are declared in parentheses:

```gf
(alice: Person { age: 30, score: 0.9 })
```

This form carries:

- a node id
- one or more labels
- optional user properties

The authored syntax is text-first, but it still maps onto the reserved node semantics Lynxes expects internally.

## Edge Form

Edges connect declared node ids:

```gf
alice -[KNOWS]-> bob
alice --[COWORKER]-- bob
```

Accepted direction forms are:

- `-[TYPE]->`
- `<-[TYPE]-`
- `<-[TYPE]->`
- `--[TYPE]--`

## Edge Property Placement

Edge properties belong after the full edge declaration:

```gf
alice -[KNOWS]-> bob { since: 2020, weight: 0.9 }
```

Do not place edge properties inside the bracketed edge-type segment. That is a common source of parse failure.

## Comments

Use `#` for single-line comments.

## Authoring-Related Features

`.gf` also supports a broader authored file structure than the tiny examples on this page show. In practice that can include:

- `@meta` blocks
- `node` schema definitions
- `edge` schema definitions
- schema inheritance on the node side
- typed property declarations and directives

Those features are intentionally not expanded in detail here because this page is the short reference version. They are covered more fully in the companion [`.gf` authoring guide](../../gf_authoring_guide.md).

## Validation Notes

Typical `.gf` failures include:

- malformed node declarations
- malformed edge declarations
- misplaced edge properties
- duplicate node ids
- edges that reference nodes that were never declared
- use of reserved graph-column names as authored user properties

For a broader writing guide, use the `.gf` authoring guide. This page is meant to stay closer to the accepted format shape and validation rules.
