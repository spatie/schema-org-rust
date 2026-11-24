---
title: Generating types
weight: 7
---

The Python standard-library generator reads the [Schema.org 30.0 JSON-LD definitions](https://raw.githubusercontent.com/schemaorg/schemaorg/main/data/releases/30.0/schemaorg-all-https.jsonld). A copy is committed under `generator/`, so normal regeneration is offline and needs only Python 3 and `rustfmt`.

```sh
python3 generator/generate.py
```

This writes `src/generated.rs` and `tests/vocabulary.rs`. It generates all Schema.org classes except primitive data types, every property setter, enumeration constants, and type accessors on `Schema`, `Graph`, and `MultiTypedEntity`. Inherited properties work on every builder. Unknown extension properties can still be assigned using `set_property`.

Names use Rust conventions: `localBusiness` becomes `local_business`; enumeration values become `SCREAMING_SNAKE_CASE`; keywords gain a trailing underscore. `3DModel` is exposed as `ThreeDimensionalModel`.

To download the pinned upstream source again:

```sh
python3 generator/generate.py --refresh
```

To adopt a newer vocabulary, change the pinned source URL and version in the generator, refresh, regenerate, and review the diff. Run the quality checks in [testing](testing). The generated coverage tests exercise every factory, property mapping, enumeration URL, and collection accessor. CI also regenerates the files and rejects drift.

The vendored vocabulary is from the Schema.org project under its Apache 2.0 license, included at `generator/LICENSE-schemaorg`. The Rust implementation is MIT licensed.
