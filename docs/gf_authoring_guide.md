# Lynxes `.gf` Format Authoring Guide

The `.gf` file is a human-authorable text format designed for representing graph data natively in Lynxes.
This document serves as a spec and authoring guide summarizing the syntax rules for writing `.gf` files.

For detailed parsing rules and internal specifications, refer to [Engineering Spec: `.gf` Text Format Specification](spec/gf_format.md).

---

## 1. Basic Structure

A `.gf` file is typically written in the following order:
1. File-level control blocks such as `@meta` (Optional)
2. `node` and `edge` schema definitions (Optional)
3. Node declarations (Required)
4. Edge declarations (Required)

> **Note:** An empty file, or a file containing only metadata/schemas is invalid. You must declare at least one node or edge.

---

## 2. Declaring Nodes

Nodes are enclosed in parentheses `()`. The first identifier inside the parentheses defines the node's local unique ID.

### Basic Node
```gf
(alice)
```

### Adding Labels
Specify labels after a colon `:`. You can assign multiple labels using the `|` delimiter.
```gf
(alice: Person)
(bob: Person|Employee)
```

### Adding Properties
Define properties inside curly braces `{}` using a `key: value` format. Separate multiple properties with commas `,`.
```gf
(alice: Person { name: "Alice", age: 30, city: "Seoul" })
```

---

## 3. Declaring Edges

Edges represent the connection and its directionality using an arrow-like syntax. The edge type must be specified inside brackets `[]`. The endpoints of the edge reference the unique node IDs declared in the parentheses.

### Edge Directions
- **Outbound (Forward):** `-[TYPE]->`
- **Inbound (Reverse):** `<-[TYPE]-`
- **Bidirectional:** `<-[TYPE]->`
- **Undirected:** `--[TYPE]--`

### Examples
```gf
# Outbound edge (alice knows bob)
alice -[KNOWS]-> bob

# Bidirectional / Undirected edges
alice <-[FRIENDS]-> bob
alice --[COWORKER]-- bob
```

### Adding Edge Properties
Edge properties are added using curly braces `{}` **after** the entire edge declaration.
```gf
alice -[KNOWS]-> bob { since: 2020, weight: 0.9 }
```
> **Note:** Inline bracket properties like `alice -[KNOWS { since: 2020 }]-> bob` are NOT valid syntax.

---

## 4. Defining Schemas

Schemas can be defined to explicitly validate and control the structure of `node` and `edge` data.

### Node Schema
```gf
node Person {
    name: String @index
    age: Int?
}

node Employee extends Person {
    employee_id: String @unique
}
```
* **Inheritance**: You can inherit from another node schema using the `extends` keyword.
* **Optional Properties**: Append `?` after the type to denote an optional property.
* **Directives**: Append directives like `@index`, `@unique`, or `@default(value)` to define control bounds.

### Edge Schema
```gf
edge KNOWS {
    since: Int?
    weight: Float @default(1.0)
}
```
* Edge schemas do NOT support inheritance (`extends`).

---

## 5. Metadata and Other Directives

Metadata blocks are used to include file-level meta information. It is recommended to place them at the top of the file.

```gf
@meta {
    name: "social_graph",
    version: "1.0",
    created: 2026-04-20
}
```

---

## 6. Supported Data Types and Literals

When writing values in the `.gf` format, use the following literal rules:

* **String:** Wrapped in double quotes e.g., `"Hello", "Alice"`
* **Int:** Base-10 integers e.g., `42`, `-7`
* **Float:** Numbers with a decimal point e.g., `3.14`, `-0.5`
* **Bool:** `true`, `false` (Must be lowercase)
* **Date:** `YYYY-MM-DD` format (e.g., `2026-04-20`)
* **DateTime:** `YYYY-MM-DDTHH:MM:SS` format
* **List:** Wrapped in square brackets e.g., `[1, 2, 3]`, `["a", "b"]`
* **Null:** `null` (Must be lowercase)

---

## 7. Recommended Style (Best Practices)

```gf
# 1. Define metadata at the top of the file
@meta {
    name: "employee_graph",
    version: "1.0"
}

# 2. Define schemas for data formatting
node Person {
    name: String
    age: Int?
}
node Company {
    name: String
}
edge WORKS_AT {
    role: String
    since: Int
}

# 3. List Node Instances
(alice: Person { name: "Alice", age: 30 })
(bob: Person { name: "Bob", age: 25 })
(acme: Company { name: "Acme Corp" })

# 4. List Connected Edges
alice -[WORKS_AT]-> acme { role: "Engineer", since: 2023 }
bob -[WORKS_AT]-> acme { role: "Designer", since: 2024 }
alice -[KNOWS]-> bob {}
```

---

## 8. Constraints & Caveats

* **Reserved Words:** Properties starting with an underscore `_` (e.g., `_id`, `_label`, `_src`, `_dst`, `_type`, `_direction`) are reserved for internal system columns and cannot be authored manually.
* **Encoding:** The file MUST be saved with **UTF-8 encoding**.
* **Parsing Rules:** Whitespace and indentation do not affect parsing. However, for readability, it is highly recommended to use consistent indentation and place properties cleanly.
* **Comments:** Use `#` for single-line comments.
