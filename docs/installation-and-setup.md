---
title: Installation
weight: 3
---

Requires Rust 1.89 or newer. The crate uses edition 2024 and has no runtime services or PHP dependencies.

The crate has not been published to crates.io. Add it from Git:

```toml
[dependencies]
spatie-schema-org = { git = "https://github.com/spatie/schema-org-rust", branch = "main" }
```

For a local checkout, use a path dependency instead:

```toml
[dependencies]
spatie-schema-org = { path = "../schema-org-rust" }
```

Import `Schema` for factories and `JsonLd` for output methods. All generated builders and enumeration types are available at the crate root.

```rust
use spatie_schema_org::{JsonLd, Person, Schema};

let mut person: Person = Schema::person();
person.name("Freek");
assert_eq!(person.to_value()["@type"], "Person");
```
