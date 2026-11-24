---
title: Multi-typed entities
weight: 6
---

Schema.org permits a single entity to have several types. `MultiTypedEntity` merges component builders into one JSON-LD object with an `@type` array.

```rust
use spatie_schema_org::{JsonLd, MultiTypedEntity, Schema};

let mut room = MultiTypedEntity::new();
room.hotel_room().name("The Presidential Suite");
room.product().offers(
    Schema::offer().name("One Night").price(100000).price_currency("USD")
);
room.product().aggregate_rating(
    Schema::aggregate_rating().best_rating(5).worst_rating(4)
);

assert_eq!(room.to_value()["@type"], serde_json::json!(["HotelRoom", "Product"]));
assert_eq!(room.to_value()["offers"]["priceCurrency"], "USD");
```

Repeated calls to an accessor return the same component. Components merge in insertion order, with later components winning property conflicts. Each type appears once. An empty collection renders an empty `@type` array.

`add` rejects an existing type with `CollectionError::AlreadyExists`. `set` inserts or replaces a component, keeping its position. `has` checks membership. `get` returns a shared component handle or `CollectionError::NotFound`. `get_or_create` accepts a vocabulary type name and returns `CollectionError::InvalidType` for unknown names. `get_nodes` returns all component handles.

```rust
# use spatie_schema_org::{JsonLd, MultiTypedEntity, Schema};
let mut entity = MultiTypedEntity::new();
entity.add(Schema::hotel_room().name("Suite"))?;
entity.set(Schema::product().name("Suite booking"));
assert!(entity.has("HotelRoom"));
assert_eq!(entity.to_value()["name"], "Suite booking");
entity.when(true, |entity| { entity.product().description("One night"); });
entity.set_nonce("your-csp-nonce");
assert!(entity.to_script().contains("nonce=\"your-csp-nonce\""));
# Ok::<(), spatie_schema_org::CollectionError>(())
```

Multi-typed entities can be nested in other builders. Nested output omits `@context`. `to_value`, `to_json`, `to_script`, `Display`, and Serde serialization work just as they do for a single type.
