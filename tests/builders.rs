//! Observable builder behavior ported from the PHP package's public contract.

use std::collections::BTreeMap;

use serde_json::{Value, json};
use spatie_schema_org::{
    BookFormatType, Entity, Graph, IntoProperty, JsonLd, MultiTypedEntity, Schema,
};

#[test]
fn fluent_builders_preserve_nested_live_entities_and_arrays() {
    let mut contact = Schema::contact_point();
    let mut business = Schema::local_business();
    business
        .name("Spatie")
        .email("info@spatie.be")
        .contact_point(&contact)
        .same_as(["https://spatie.be", "https://github.com/spatie"]);
    contact.area_served("Worldwide");
    assert_eq!(
        business.to_value(),
        json!({
            "@context": "https://schema.org", "@type": "LocalBusiness", "name": "Spatie",
            "email": "info@spatie.be", "contactPoint": {"@type": "ContactPoint", "areaServed": "Worldwide"},
            "sameAs": ["https://spatie.be", "https://github.com/spatie"]
        })
    );
}

#[test]
fn property_operations_ignore_only_null_and_empty_strings() {
    let mut person = Schema::person();
    person
        .name("Freek")
        .name("")
        .email(None::<&str>)
        .add_properties([
            ("zero", json!(0)),
            ("false", json!(false)),
            ("empty", json!([])),
        ])
        .when(false, |p| {
            p.name("wrong");
        })
        .when(true, |p| {
            p.set_property("custom:field", "value");
        });
    assert_eq!(
        person.get_properties(),
        json!({"name":"Freek", "zero":0, "false":false, "empty":[], "custom:field":"value"})
            .as_object()
            .unwrap()
            .clone()
    );
    assert!(person.remove_property("name"));
    assert!(!person.remove_property("name"));
    assert_eq!(person.get_property("name"), None);
}

#[test]
fn arrays_and_maps_keep_nested_builders_and_literal_values() {
    let mut author = Schema::person();
    let mut article = Schema::article();
    article.author([&author]).set_property(
        "metadata",
        BTreeMap::from([
            ("child", (&author).into_property()),
            ("literal", "text".into_property()),
        ]),
    );
    author.name("Zoë");
    assert_eq!(
        article.to_value()["metadata"],
        json!({"child":{"@type":"Person", "name":"Zoë"}, "literal":"text"})
    );
    assert_eq!(
        article.to_value()["author"],
        json!([{"@type":"Person", "name":"Zoë"}])
    );
}

#[test]
fn identifiers_preserve_typed_values_but_promote_other_values() {
    let mut property = Schema::property_value();
    property.identifier("#1").value("123");
    let mut product = Schema::product();
    product
        .identifier(&property)
        .set_property("@id", "#product");
    assert_eq!(
        product.to_value(),
        json!({"@context":"https://schema.org", "@type":"Product", "@id":"#product",
        "identifier":{"@type":"PropertyValue", "@id":"#1", "value":"123"}})
    );
    product.identifier("#new");
    assert_eq!(
        product.to_value(),
        json!({"@context":"https://schema.org", "@type":"Product", "@id":"#new"})
    );
}

#[test]
fn cyclic_references_resolve_identifiers_assigned_after_linking() {
    let mut alice = Schema::person();
    let mut bob = Schema::person();
    alice.knows(bob.referenced());
    bob.knows(alice.referenced());
    alice.identifier("#alice");
    bob.identifier("#bob");
    assert_eq!(
        alice.to_value(),
        json!({"@context":"https://schema.org", "@type":"Person", "@id":"#alice", "knows":{"@id":"#bob"}})
    );
    assert_eq!(bob.to_value()["knows"], json!({"@id":"#alice"}));
}

#[test]
fn references_use_explicit_ids_and_render_the_original_script() {
    let mut entity = Schema::person();
    entity
        .identifier(Schema::property_value())
        .set_property("@id", "#explicit")
        .set_nonce("nonce");
    let reference = entity.referenced();
    assert_eq!(reference.to_value(), json!({"@id":"#explicit"}));
    assert_eq!(reference.to_script(), entity.to_script());
    entity.remove_property("@id");
    assert_eq!(reference.to_value(), json!({"@id":null}));
}

#[test]
fn references_do_not_keep_cyclic_targets_alive() {
    let reference = {
        let entity = Schema::person();
        entity.referenced()
    };
    assert_eq!(reference.to_value(), json!({"@id":null}));
}

#[test]
fn datetimes_use_atom_format_and_dates_use_iso8601() {
    let mut event = Schema::event();
    event
        .start_date(
            time::OffsetDateTime::parse(
                "2026-11-24T10:23:45.123+01:00",
                &time::format_description::well_known::Rfc3339,
            )
            .unwrap(),
        )
        .end_date(time::Date::from_calendar_date(2026, time::Month::November, 25).unwrap());
    assert_eq!(event.to_value()["startDate"], "2026-11-24T10:23:45+01:00");
    assert_eq!(event.to_value()["endDate"], "2026-11-25");
    event.start_date(
        time::OffsetDateTime::parse(
            "2026-11-24T10:23:45-03:30",
            &time::format_description::well_known::Rfc3339,
        )
        .unwrap(),
    );
    assert_eq!(event.to_value()["startDate"], "2026-11-24T10:23:45-03:30");
    event.start_date(time::OffsetDateTime::UNIX_EPOCH);
    assert_eq!(event.to_value()["startDate"], "1970-01-01T00:00:00+00:00");
}

#[test]
fn enumeration_values_are_schema_urls() {
    assert_eq!(
        Schema::book()
            .book_format(BookFormatType::HARDCOVER)
            .to_value()["bookFormat"],
        "https://schema.org/Hardcover"
    );
}

#[test]
fn scripts_escape_html_without_corrupting_json_and_escape_nonce_attributes() {
    let mut entity = Schema::thing();
    entity
        .name("</script><script>alert('Zoë & 😀')</script>")
        .set_nonce("a\"'&<>");
    let script = entity.to_script();
    assert!(
        script.starts_with(
            "<script type=\"application/ld+json\" nonce=\"a&quot;&#039;&amp;&lt;&gt;\">"
        )
    );
    assert_eq!(script.matches("</script>").count(), 1);
    let json = script
        .split_once("\">")
        .unwrap()
        .1
        .strip_suffix("</script>")
        .unwrap();
    assert_eq!(
        serde_json::from_str::<Value>(json).unwrap(),
        entity.to_value()
    );
    assert_eq!(entity.to_string(), script);
    assert_eq!(serde_json::to_value(&entity).unwrap(), entity.to_value());
}

#[test]
fn graph_links_are_live_even_if_the_helper_node_is_hidden() {
    let mut graph = Graph::new();
    let organization = graph.organization().clone();
    graph.product().name("My cool Product").brand(&organization);
    graph.hide("Organization", None);
    graph.organization().name("My awesome Company");
    assert_eq!(
        graph.to_value(),
        json!({"@context":"https://schema.org", "@graph":[
            {"@type":"Product", "name":"My cool Product", "brand":{"@type":"Organization", "name":"My awesome Company"}}
        ]})
    );
}

#[test]
fn graph_keys_do_not_become_jsonld_identifiers_and_nodes_are_grouped_by_type() {
    let mut graph = Graph::new();
    graph
        .get_or_create("Person", "freek")
        .unwrap()
        .given_name("Freek");
    graph.organization().name("Spatie");
    graph
        .get_or_create("Person", "sebastian")
        .unwrap()
        .given_name("Sebastian");
    graph
        .get_or_create("Person", "freek")
        .unwrap()
        .family_name("Van der Herten");
    assert_eq!(
        graph.to_value()["@graph"],
        json!([
            {"@type":"Person", "givenName":"Freek", "familyName":"Van der Herten"},
            {"@type":"Person", "givenName":"Sebastian"}, {"@type":"Organization", "name":"Spatie"}
        ])
    );
}

#[test]
fn graph_enforces_unique_keys_but_set_replaces_and_remove_deletes() {
    let mut graph = Graph::new();
    let mut alice = Schema::person();
    alice.name("Alice");
    graph.add(&alice, "a").unwrap();
    assert!(matches!(
        graph.add(&alice, "a"),
        Err(spatie_schema_org::CollectionError::AlreadyExists(_, _))
    ));
    assert!(matches!(
        graph.get("Person", "missing"),
        Err(spatie_schema_org::CollectionError::NotFound(_, _))
    ));
    assert!(matches!(
        graph.get_or_create("Missing", "a"),
        Err(spatie_schema_org::CollectionError::InvalidType(_))
    ));
    let mut bob = Schema::person();
    bob.name("Bob");
    graph.set(&bob, "a");
    assert_eq!(
        graph.get("Person", "a").unwrap().get_property("name"),
        Some(json!("Bob"))
    );
    assert!(graph.remove("Person", "a").is_some());
    assert!(!graph.has("Person", "a"));
}

#[test]
fn graph_visibility_handles_all_single_future_and_restored_nodes() {
    let mut graph = Graph::new();
    for key in ["a", "b", "c"] {
        graph.get_or_create("Person", key).unwrap().name(key);
    }
    graph.hide("Person", None).hide("Person", Some("b"));
    assert_eq!(graph.to_value()["@graph"], json!([]));
    graph.show("Person", Some("b"));
    assert_eq!(
        graph.to_value()["@graph"],
        json!([{"@type":"Person", "name":"b"}])
    );
    graph.show("Person", None).hide("Person", Some("c"));
    assert_eq!(graph.to_value()["@graph"].as_array().unwrap().len(), 2);
    graph.show("Person", Some("c"));
    assert_eq!(graph.to_value()["@graph"].as_array().unwrap().len(), 3);
    graph.hide("Organization", Some("default"));
    graph.organization().name("hidden");
    assert_eq!(graph.to_value()["@graph"].as_array().unwrap().len(), 3);
    assert!(graph.has("Organization", "default"));
}

#[test]
fn graph_context_and_serialization_work_for_empty_graphs_and_nonces() {
    let context = json!(["https://schema.org", {"@base":"https://spatie.be/"}]);
    let mut graph = Graph::with_context(context.clone());
    graph.set_nonce("\"<").when(false, |g| {
        g.person();
    });
    assert_eq!(
        serde_json::to_value(&graph).unwrap(),
        json!({"@context":context, "@graph":[]})
    );
    assert!(graph.to_script().contains("nonce=\"&quot;&lt;\""));
}

#[test]
fn multityped_entities_merge_in_order_and_keep_nested_contexts_out() {
    let mut mte = MultiTypedEntity::new();
    mte.hotel_room().name("The Presidential Suite");
    mte.product()
        .name("Suite booking")
        .offers(Schema::offer().price(100000).price_currency("USD"));
    mte.product()
        .aggregate_rating(Schema::aggregate_rating().best_rating(5).worst_rating(4));
    assert_eq!(
        mte.to_value(),
        json!({"@context":"https://schema.org", "@type":["HotelRoom", "Product"],
        "name":"Suite booking", "offers":{"@type":"Offer", "price":100000, "priceCurrency":"USD"},
        "aggregateRating":{"@type":"AggregateRating", "bestRating":5, "worstRating":4}})
    );
    assert_eq!(serde_json::to_value(&mte).unwrap(), mte.to_value());
}

#[test]
fn multityped_entities_enforce_uniqueness_and_replace_types_in_place() {
    let mut mte = MultiTypedEntity::new();
    assert_eq!(
        mte.to_value(),
        json!({"@context":"https://schema.org", "@type":[]})
    );
    mte.add(&Schema::product()).unwrap();
    assert!(mte.add(&Schema::product()).is_err());
    assert!(mte.get("HotelRoom").is_err());
    mte.set(Schema::product().name("replacement"));
    mte.get_or_create("HotelRoom").unwrap().name("later");
    assert_eq!(mte.to_value()["@type"], json!(["Product", "HotelRoom"]));
    assert_eq!(mte.to_value()["name"], "later");
    mte.set_nonce("nonce");
    assert!(mte.to_script().contains("nonce=\"nonce\""));
}

#[test]
fn custom_types_and_properties_remain_available_without_generated_names() {
    let mut entity: Entity = Entity::custom("CustomType");
    entity.set_property("customProperty", 17);
    assert_eq!(
        entity.to_value(),
        json!({"@context":"https://schema.org", "@type":"CustomType", "customProperty":17})
    );
}

#[test]
fn nested_collections_include_subsequent_membership_changes() {
    let mut multi = MultiTypedEntity::new();
    let mut graph = Graph::new();
    let mut page = Schema::web_page();
    page.main_entity(&multi).set_property("related", &graph);
    multi.hotel_room().name("Suite");
    multi.product().sku("A-123");
    graph.organization().name("Spatie");
    assert_eq!(
        page.to_value()["mainEntity"],
        json!({"@type":["HotelRoom", "Product"], "name":"Suite", "sku":"A-123"})
    );
    assert_eq!(
        page.to_value()["related"],
        json!({"@graph":[{"@type":"Organization", "name":"Spatie"}]})
    );
}

#[test]
fn reserved_jsonld_fields_cannot_override_builder_context_or_type() {
    let mut child = Schema::person();
    child
        .set_property("@context", "https://invalid.example")
        .set_property("@type", "WrongType");
    let mut page = Schema::web_page();
    page.main_entity(&child);
    assert_eq!(
        child.to_value(),
        json!({"@context":"https://schema.org", "@type":"Person"})
    );
    assert_eq!(page.to_value()["mainEntity"], json!({"@type":"Person"}));
}
