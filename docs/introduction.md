---
title: Introduction
weight: 1
---

This package provides fluent builders for every Schema.org type and their properties, with JSON-LD output. It is a Rust port of [spatie/schema-org](https://github.com/spatie/schema-org).

The code is generated from Schema.org's 30.0 JSON-LD definitions. It includes 925 types, 1,529 property setters, and 536 enumeration values.

```rust
use spatie_schema_org::{JsonLd, Schema};

let mut business = Schema::local_business();
business.name("Spatie")
    .email("info@spatie.be")
    .contact_point(Schema::contact_point().area_served("Worldwide"));

println!("{}", business.to_script());
# assert_eq!(business.to_value()["name"], "Spatie");
```

Builders support nested types, arrays, custom properties, identifiers and references, graphs, and multi-typed entities. Output implements Serde's `Serialize`. Script output escapes HTML delimiters and accepts a CSP nonce.

Continue with [installation](installation-and-setup) and [usage](usage).
