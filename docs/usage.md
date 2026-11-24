---
title: Usage
weight: 4
---

## Creating types

All types can be created through `Schema` or their own `new` constructor. Setters use snake_case. They mutate the builder and return `&mut Self` for chaining.

```rust
use spatie_schema_org::{JsonLd, LocalBusiness, Schema};

let mut business = Schema::local_business();
business.name("Spatie").email("info@spatie.be");

let mut another = LocalBusiness::new();
another.name("Spatie");
assert_eq!(another.to_value()["name"], "Spatie");
```

Store a builder in a mutable variable when retaining it after a chain. Nested builders can be passed by value, shared reference, or mutable reference. Arrays, vectors, and slices work for properties with multiple values.

```rust
# use spatie_schema_org::{JsonLd, Schema};
let mut contact = Schema::contact_point();
let mut business = Schema::local_business();
business.contact_point(&contact).same_as(["https://spatie.be", "https://github.com/spatie"]);
contact.area_served("Worldwide");
assert_eq!(business.to_value()["contactPoint"]["areaServed"], "Worldwide");
```

Clones share the same entity, including later changes. Graphs and multi-typed entities also share their contents when cloned or nested. Builders are single-threaded. Serialize to an owned `serde_json::Value` before sending data across threads. Use identifier references rather than nesting an entity into itself or constructing a nesting cycle.

The API permits every generated property on every entity. It does not validate Schema.org domain/range rules. Refer to [schema.org](https://schema.org) and validate the final document with Google's [Rich Results Test](https://search.google.com/test/rich-results).

## Output

`to_value` returns an owned JSON value. `to_json` returns compact JSON. `to_script` and `Display` return an HTML script tag. Nested builders omit their own `@context`.

```rust
# use spatie_schema_org::{JsonLd, Schema};
let mut business = Schema::local_business();
business.name("Spatie").set_nonce("your-csp-nonce");
let json = business.to_json();
let script = business.to_script();
assert!(script.starts_with("<script type=\"application/ld+json\" nonce=\"your-csp-nonce\">"));
assert_eq!(business.to_string(), script);
assert_eq!(serde_json::to_value(&business).unwrap(), business.to_value());
```

Script output escapes `<` and `>` in JSON and HTML-escapes the nonce attribute. Plain JSON remains plain JSON; use `to_script` when embedding it in HTML.

## Enumerations and dates

Enumeration values are associated `SCREAMING_SNAKE_CASE` string constants containing Schema.org URLs. Rust keywords use a trailing underscore in generated method names, for example `box_`.

```rust
use spatie_schema_org::{BookFormatType, JsonLd, Schema};

let mut book = Schema::book();
book.book_format(BookFormatType::HARDCOVER);
assert_eq!(book.to_value()["bookFormat"], "https://schema.org/Hardcover");
```

Pass dates as ISO 8601 strings or `time::Date` values. `time::OffsetDateTime` values include the timezone offset and whole seconds. Add `time` with its `parsing` feature to your own dependencies when parsing date strings as in this example.

```rust
# use spatie_schema_org::{JsonLd, Schema};
let mut event = Schema::event();
let date = time::OffsetDateTime::parse("2026-11-24T10:00:00+01:00", &time::format_description::well_known::Rfc3339).unwrap();
event.start_date(date);
assert_eq!(event.to_value()["startDate"], "2026-11-24T10:00:00+01:00");
```

## Custom properties and conditional changes

`set_property` accepts any property name. `add_properties` accepts an iterator of name/value pairs. `get_property` returns `Option<Value>`; use `unwrap_or` to provide a default. `get_properties` returns the explicitly assigned properties and `remove_property` removes one.

```rust
# use spatie_schema_org::{JsonLd, Schema};
let mut business = Schema::local_business();
business.add_properties([("name", "Spatie"), ("foo", "bar")])
    .when(true, |schema| { schema.email("info@spatie.be"); })
    .when(false, |schema| { schema.name("not used"); });
assert_eq!(business.get_property("foo"), Some(serde_json::json!("bar")));
assert!(business.remove_property("foo"));
assert_eq!(business.get_type(), "LocalBusiness");
assert_eq!(business.get_context(), "https://schema.org");
```

Null (`None` or `Value::Null`) and empty strings are ignored, including when a property already exists. Zero, false, and empty arrays are retained. JSON objects can be passed using `serde_json::json!`; `BTreeMap` also accepts nested builders. Convert heterogeneous values with `IntoProperty::into_property`. For an extension type, use `let mut entity: Entity = Entity::custom("MyType");`.

## Identifiers and references

Simple `identifier` values render as `@id`. A builder passed as an identifier stays under `identifier`, for example a `PropertyValue`.

```rust
# use spatie_schema_org::{JsonLd, Schema};
let mut company = Schema::organization();
let mut person = Schema::person();
person.works_for(company.referenced());
company.identifier("https://spatie.be/#organization").name("Spatie");
person.identifier("#freek");
assert_eq!(person.to_value()["@id"], "#freek");
assert_eq!(person.to_value()["worksFor"], serde_json::json!({"@id":"https://spatie.be/#organization"}));
```

References resolve the target identifier at render time, including identifiers assigned after linking. They serialize only `@id`, so two entities can reference each other without recursive output. A reference's script output renders the target's complete document.

References use weak ownership to avoid leaking cycles. Keep the target builder, or a graph owning it, alive until rendering. A dropped target or absent identifier produces `{"@id":null}`. Use `.referenced()` on a retained builder, not on a temporary that is immediately dropped.

See [graphs](graphs) for several nodes and [multi-typed entities](multi-typed-entities) for one node with several types.
