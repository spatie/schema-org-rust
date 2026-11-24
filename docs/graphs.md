---
title: Graphs
weight: 5
---

A `Graph` collects Schema.org nodes and renders them under one `@context` and `@graph`. Generated accessors get or create a default node. Each accessor returns a shared handle, so you can keep and link nodes without borrowing the graph.

```rust
use spatie_schema_org::{Graph, JsonLd};

let mut graph = Graph::new();
let company = graph.organization();
graph.product().name("My cool Product").brand(&company);
graph.hide("Organization", None);
graph.organization().name("My awesome Company");

assert_eq!(graph.to_value()["@graph"][0]["brand"]["name"], "My awesome Company");
assert_eq!(graph.to_value()["@graph"].as_array().unwrap().len(), 1);
```

Hidden nodes remain available for nesting or references but do not appear as top-level graph entries. `hide("Person", Some("key"))` hides one node. `hide("Person", None)` hides every node of that type, including future nodes. `show` reverses these operations. Showing a single node after hiding its type keeps the other existing nodes hidden.

## Several nodes of the same type

Use `get_or_create` with a type name and local key. Generated accessors use the local key `"default"`. Local keys are collection keys, not JSON-LD identifiers; use `identifier` to assign `@id`.

```rust
use spatie_schema_org::{Graph, JsonLd};

let mut graph = Graph::new();
graph.get_or_create("Person", "freekmurze")?
    .given_name("Freek").family_name("Van der Herten").alternate_name("freekmurze");
graph.get_or_create("Person", "sebastiandedeyne")?
    .given_name("Sebastian").family_name("De Deyne");
graph.get_or_create("Person", "random")?.name("Random Person");
graph.hide("Person", Some("random"));
assert_eq!(graph.to_value()["@graph"].as_array().unwrap().len(), 2);
# Ok::<(), spatie_schema_org::CollectionError>(())
```

Nodes render grouped by type in first-insertion order, then by local key insertion order. Setting a node again preserves its position.

## Managing nodes

`has` checks a type/key pair. `get` returns a shared node handle or `CollectionError::NotFound`. `add` rejects duplicates with `CollectionError::AlreadyExists`; `set` inserts or replaces. `remove` deletes a node. `get_nodes` returns shared handles with their local keys, including hidden nodes.

```rust
# use spatie_schema_org::{Graph, Schema};
let mut graph = Graph::new();
graph.add(Schema::person().name("Freek"), "freek")?;
assert!(graph.has("Person", "freek"));
graph.get("Person", "freek")?.email("freek@spatie.be");
assert!(graph.remove("Person", "freek").is_some());
# Ok::<(), spatie_schema_org::CollectionError>(())
```

Unknown names passed to `get_or_create` return `CollectionError::InvalidType`. Custom types can be inserted with `set`. `when` accepts a conditional callback for graph changes; use ordinary Rust blocks for callbacks that need several node handles.

## Custom contexts and CSP

```rust
use spatie_schema_org::{Graph, JsonLd};

let mut graph = Graph::with_context(serde_json::json!([
    "https://schema.org", {"@base":"https://spatie.be/"}
]));
graph.set_nonce("your-csp-nonce");
assert_eq!(graph.get_context()[1]["@base"], "https://spatie.be/");
assert!(graph.to_script().contains("nonce=\"your-csp-nonce\""));
```

Graphs support `to_value`, `to_json`, `to_script`, `Display`, and Serde serialization, including empty graphs.
