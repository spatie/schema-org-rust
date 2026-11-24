# A Fluent Builder For Schema.org Types And ld+json Generator

[![Tests](https://img.shields.io/github/actions/workflow/status/spatie/schema-org-rust/ci.yml?branch=main&style=flat-square)](https://github.com/spatie/schema-org-rust/actions/workflows/ci.yml)
[![License](https://img.shields.io/badge/license-MIT-blue.svg?style=flat-square)](https://github.com/spatie/schema-org-rust/blob/main/LICENSE.md)

Fluent builders for every Schema.org type and JSON-LD output. A Rust port of [spatie/schema-org](https://github.com/spatie/schema-org), generated from the same Schema.org 30.0 definitions: 925 types, 1,529 properties, and 536 enumeration values.

Build nested entities, link nodes by identifier, collect them in a graph, or describe one entity with multiple types. Scripts escape HTML delimiters and support CSP nonces.

## Support us

[<img src="https://spatie.be/github-ad-image/schema-org-rust.jpg" alt="Support Spatie's open source work" width="419px" />](https://spatie.be/github-ad-click/schema-org-rust)

We invest a lot of resources into creating [best in class open source packages](https://spatie.be/open-source). You can support us by [buying one of our paid products](https://spatie.be/open-source/support-us).

We highly appreciate you sending us a postcard from your hometown, mentioning which of our package(s) you are using. You'll find our address on [our contact page](https://spatie.be/about-us). We publish all received postcards on [our virtual postcard wall](https://spatie.be/open-source/postcards).

## Installation

Requires Rust 1.89 or newer (edition 2024). Add the crate to your `Cargo.toml`:

```toml
[dependencies]
spatie-schema-org = "1"
```

## Usage

```rust
use spatie_schema_org::{JsonLd, Schema};

let mut business = Schema::local_business();
business
    .name("Spatie")
    .email("info@spatie.be")
    .contact_point(Schema::contact_point().area_served("Worldwide"));

assert_eq!(business.to_value()["contactPoint"]["areaServed"], "Worldwide");
println!("{}", business.to_script());
```

Import `JsonLd` for `to_value`, `to_json`, and `to_script`. Every builder also implements Serde's `Serialize` and Rust's `Display` (script output). Setters use snake_case and return `&mut Self`. Store a builder in a mutable variable to keep using it after a chain. Clone a builder to share it, not to make a deep copy.

```rust
use spatie_schema_org::{Graph, JsonLd, MultiTypedEntity};

let mut graph = Graph::new();
let company = graph.organization();
graph.product().name("My cool Product").brand(&company);
graph.organization().name("My awesome Company");
graph.hide("Organization", None);
assert_eq!(graph.to_value()["@graph"][0]["brand"]["name"], "My awesome Company");

let mut room = MultiTypedEntity::new();
room.hotel_room().name("The Presidential Suite");
room.product().offers(spatie_schema_org::Schema::offer().price(100000).price_currency("USD"));
assert_eq!(room.to_value()["@type"][0], "HotelRoom");
```

See the [v1 documentation](https://spatie.be/docs/schema-org-rust/v1/introduction) or the [Markdown usage guide](https://github.com/spatie/schema-org-rust/blob/main/docs/usage.md).

## Generating the vocabulary

The generator and pinned JSON-LD source are committed. Generating does not require PHP or a network connection:

```sh
python3 generator/generate.py
```

Use `--refresh` to download Schema.org 30.0 again. The generator creates builders, property setters, enumeration constants, graph accessors, and vocabulary coverage tests. CI checks that generation produces no diff. See [generation details](https://github.com/spatie/schema-org-rust/blob/main/docs/generating-types.md).

## Testing

```sh
cargo test --locked --all-targets --all-features
cargo test --locked --doc --all-features
```

See [all quality checks](https://github.com/spatie/schema-org-rust/blob/main/docs/testing.md). The README and Markdown guides are tested by rustdoc, including setup on hidden `# ` lines.

## Changelog

See [CHANGELOG.md](https://github.com/spatie/schema-org-rust/blob/main/CHANGELOG.md).

## Contributing

See [CONTRIBUTING.md](https://github.com/spatie/schema-org-rust/blob/main/CONTRIBUTING.md) and the [Rust API Guidelines](https://rust-lang.github.io/api-guidelines/).

Using an AI coding agent? Load Apollo's [rust-best-practices skill](https://github.com/apollographql/skills/tree/main/skills/rust-best-practices) when writing or reviewing code in this package.

## Security Vulnerabilities

Report vulnerabilities privately using the instructions in [SECURITY.md](https://github.com/spatie/schema-org-rust/blob/main/SECURITY.md).

## Credits

- [Freek Van der Herten](https://github.com/freekmurze)
- [All contributors](https://github.com/spatie/schema-org-rust/graphs/contributors)

## License

The MIT License (MIT). See [LICENSE.md](https://github.com/spatie/schema-org-rust/blob/main/LICENSE.md).
