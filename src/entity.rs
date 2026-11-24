use std::{
    cell::RefCell,
    fmt,
    marker::PhantomData,
    rc::{Rc, Weak},
};

use serde::{Serialize, Serializer};
use serde_json::{Map, Value};

use crate::{CONTEXT, JsonLd};

/// A generated Schema.org type marker.
pub trait SchemaType {
    /// Schema.org type name.
    const NAME: &'static str;
}

#[derive(Debug)]
struct Data {
    properties: Vec<(String, Property)>,
    nonce: String,
}

/// A fluent builder with shared ownership, like the PHP builder.
///
/// Cloning shares the entity. Nested entities and graph nodes see subsequent changes.
/// Builders are single-threaded; serialize to a JSON value before sending across threads.
#[derive(Debug, Clone)]
pub struct Entity<T = crate::types::Thing> {
    pub(crate) type_name: String,
    data: Rc<RefCell<Data>>,
    marker: PhantomData<T>,
}

impl<T: SchemaType> Default for Entity<T> {
    fn default() -> Self {
        Self::new()
    }
}

impl<T: SchemaType> Entity<T> {
    /// Create an empty entity of this type.
    pub fn new() -> Self {
        Self::custom(T::NAME)
    }
}

impl<T> Entity<T> {
    /// Create a builder for a custom or extension type.
    pub fn custom(type_name: impl Into<String>) -> Self {
        Self {
            type_name: type_name.into(),
            data: Rc::new(RefCell::new(Data {
                properties: Vec::new(),
                nonce: String::new(),
            })),
            marker: PhantomData,
        }
    }

    /// Return the Schema.org type name.
    pub fn get_type(&self) -> &str {
        &self.type_name
    }

    /// Return the JSON-LD context.
    pub fn get_context(&self) -> &str {
        CONTEXT
    }

    /// Set a property, ignoring null and empty strings just like the PHP builder.
    pub fn set_property(&mut self, name: impl Into<String>, value: impl IntoProperty) -> &mut Self {
        let property = value.into_property();
        if matches!(&property.0, PropertyValue::Json(value) if value.is_null() || value == "") {
            return self;
        }
        let name = name.into();
        {
            let mut data = self.data.borrow_mut();
            if let Some((_, old)) = data.properties.iter_mut().find(|(key, _)| key == &name) {
                *old = property;
            } else {
                data.properties.push((name, property));
            }
        }
        self
    }

    /// Set multiple properties.
    pub fn add_properties<K: Into<String>, V: IntoProperty>(
        &mut self,
        properties: impl IntoIterator<Item = (K, V)>,
    ) -> &mut Self {
        for (key, value) in properties {
            self.set_property(key, value);
        }
        self
    }

    /// Get a property as an owned JSON value; absent properties return `None`.
    pub fn get_property(&self, name: &str) -> Option<Value> {
        self.data
            .borrow()
            .properties
            .iter()
            .find(|(key, _)| key == name)
            .map(|(_, value)| value.to_value())
    }

    /// Get the explicitly assigned properties as an owned JSON map.
    pub fn get_properties(&self) -> Map<String, Value> {
        self.data
            .borrow()
            .properties
            .iter()
            .map(|(key, value)| (key.clone(), value.to_value()))
            .collect()
    }

    /// Remove a property, returning whether it existed.
    pub fn remove_property(&mut self, name: &str) -> bool {
        let mut data = self.data.borrow_mut();
        if let Some(index) = data.properties.iter().position(|(key, _)| key == name) {
            data.properties.remove(index);
            true
        } else {
            false
        }
    }

    /// Apply a callback only when the condition is true.
    pub fn when(&mut self, condition: bool, callback: impl FnOnce(&mut Self)) -> &mut Self {
        if condition {
            callback(self);
        }
        self
    }

    /// Set a CSP nonce for script output.
    pub fn set_nonce(&mut self, nonce: impl Into<String>) -> &mut Self {
        self.data.borrow_mut().nonce = nonce.into();
        self
    }

    /// Reference the entity's current identifier without serializing the entity itself.
    ///
    /// Identifiers assigned later are resolved at render time. References use weak ownership
    /// to avoid leaking cyclic graphs; keep the target in a builder or graph until rendering.
    /// A missing identifier or dropped target renders as JSON null.
    pub fn referenced(&self) -> Reference {
        Reference {
            type_name: self.type_name.clone(),
            data: Rc::downgrade(&self.data),
        }
    }

    pub(crate) fn erased(&self) -> Entity {
        Entity {
            type_name: self.type_name.clone(),
            data: Rc::clone(&self.data),
            marker: PhantomData,
        }
    }

    pub(crate) fn node(&self) -> Value {
        let data = self.data.borrow();
        let mut properties: Map<String, Value> = data
            .properties
            .iter()
            .map(|(key, value)| (key.clone(), value.to_value()))
            .collect();
        if let Some((_, identifier)) = data.properties.iter().find(|(key, _)| key == "identifier")
            && !identifier.is_entity()
            && let Some(id) = properties.shift_remove("identifier")
        {
            properties.insert("@id".into(), id);
        }
        properties.shift_remove("@context");
        properties.insert("@type".into(), self.type_name.clone().into());
        Value::Object(properties)
    }
}

impl<T> JsonLd for Entity<T> {
    fn to_value(&self) -> Value {
        let mut value = self.node();
        value["@context"] = CONTEXT.into();
        value
    }
    fn nonce(&self) -> String {
        self.data.borrow().nonce.clone()
    }
}
impl<T> Serialize for Entity<T> {
    fn serialize<S: Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        self.to_value().serialize(serializer)
    }
}
impl<T> fmt::Display for Entity<T> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(&self.to_script())
    }
}

/// A weak identifier reference, resolved at render time.
#[derive(Debug, Clone)]
pub struct Reference {
    type_name: String,
    data: Weak<RefCell<Data>>,
}

impl Reference {
    fn target(&self) -> Option<Entity> {
        self.data.upgrade().map(|data| Entity {
            type_name: self.type_name.clone(),
            data,
            marker: PhantomData,
        })
    }
}
impl JsonLd for Reference {
    fn to_value(&self) -> Value {
        let id = self
            .data
            .upgrade()
            .and_then(|data| {
                let data = data.borrow();
                let find = |name| {
                    data.properties
                        .iter()
                        .find(|(key, value)| key == name && !value.is_entity())
                        .map(|(_, value)| value.to_value())
                };
                find("identifier").or_else(|| find("@id"))
            })
            .unwrap_or(Value::Null);
        serde_json::json!({"@id": id})
    }
    fn nonce(&self) -> String {
        self.target()
            .map_or_else(String::new, |target| target.nonce())
    }
    fn to_script(&self) -> String {
        self.target()
            .map_or_else(String::new, |target| target.to_script())
    }
}
impl Serialize for Reference {
    fn serialize<S: Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        self.to_value().serialize(serializer)
    }
}
impl fmt::Display for Reference {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(&self.to_script())
    }
}

/// A property value, preserving live nested builders until serialization.
#[derive(Debug, Clone)]
pub struct Property(PropertyValue);

#[derive(Debug, Clone)]
enum PropertyValue {
    Json(Value),
    Entity(Entity),
    Reference(Reference),
    Graph(crate::Graph),
    Multi(crate::MultiTypedEntity),
    Array(Vec<Property>),
    Object(Vec<(String, Property)>),
}
impl Property {
    fn is_entity(&self) -> bool {
        matches!(
            self.0,
            PropertyValue::Entity(_)
                | PropertyValue::Reference(_)
                | PropertyValue::Graph(_)
                | PropertyValue::Multi(_)
        )
    }
    pub(crate) fn to_value(&self) -> Value {
        match &self.0 {
            PropertyValue::Json(value) => value.clone(),
            PropertyValue::Entity(entity) => entity.node(),
            PropertyValue::Reference(reference) => reference.to_value(),
            PropertyValue::Graph(graph) => {
                let mut value = graph.to_value();
                if let Value::Object(map) = &mut value {
                    map.shift_remove("@context");
                }
                value
            }
            PropertyValue::Multi(multi) => {
                let mut value = multi.to_value();
                if let Value::Object(map) = &mut value {
                    map.shift_remove("@context");
                }
                value
            }
            PropertyValue::Array(values) => values.iter().map(Self::to_value).collect(),
            PropertyValue::Object(values) => Value::Object(
                values
                    .iter()
                    .map(|(key, value)| (key.clone(), value.to_value()))
                    .collect(),
            ),
        }
    }
}

/// Convert values into JSON-LD properties, omitting nested entity contexts.
pub trait IntoProperty {
    /// Convert this value into an owned property or shared builder handle.
    fn into_property(self) -> Property;
}
impl IntoProperty for Property {
    fn into_property(self) -> Property {
        self
    }
}
impl<T> IntoProperty for Entity<T> {
    fn into_property(self) -> Property {
        Property(PropertyValue::Entity(self.erased()))
    }
}
impl<T> IntoProperty for &Entity<T> {
    fn into_property(self) -> Property {
        Property(PropertyValue::Entity(self.erased()))
    }
}
impl<T> IntoProperty for &mut Entity<T> {
    fn into_property(self) -> Property {
        Property(PropertyValue::Entity(self.erased()))
    }
}
impl IntoProperty for Reference {
    fn into_property(self) -> Property {
        Property(PropertyValue::Reference(self))
    }
}
impl IntoProperty for &Reference {
    fn into_property(self) -> Property {
        self.clone().into_property()
    }
}
impl IntoProperty for crate::Graph {
    fn into_property(self) -> Property {
        Property(PropertyValue::Graph(self))
    }
}
impl IntoProperty for &crate::Graph {
    fn into_property(self) -> Property {
        self.clone().into_property()
    }
}
impl IntoProperty for &mut crate::Graph {
    fn into_property(self) -> Property {
        self.clone().into_property()
    }
}
impl IntoProperty for crate::MultiTypedEntity {
    fn into_property(self) -> Property {
        Property(PropertyValue::Multi(self))
    }
}
impl IntoProperty for &crate::MultiTypedEntity {
    fn into_property(self) -> Property {
        self.clone().into_property()
    }
}
impl IntoProperty for &mut crate::MultiTypedEntity {
    fn into_property(self) -> Property {
        self.clone().into_property()
    }
}
impl<V: IntoProperty> IntoProperty for Option<V> {
    fn into_property(self) -> Property {
        self.map_or_else(|| Value::Null.into_property(), IntoProperty::into_property)
    }
}
impl<V: IntoProperty> IntoProperty for Vec<V> {
    fn into_property(self) -> Property {
        Property(PropertyValue::Array(
            self.into_iter().map(IntoProperty::into_property).collect(),
        ))
    }
}
impl<V: IntoProperty, const N: usize> IntoProperty for [V; N] {
    fn into_property(self) -> Property {
        Property(PropertyValue::Array(
            self.into_iter().map(IntoProperty::into_property).collect(),
        ))
    }
}
impl<V: IntoProperty + Clone> IntoProperty for &[V] {
    fn into_property(self) -> Property {
        self.to_vec().into_property()
    }
}
impl<K: Into<String>, V: IntoProperty> IntoProperty for std::collections::BTreeMap<K, V> {
    fn into_property(self) -> Property {
        Property(PropertyValue::Object(
            self.into_iter()
                .map(|(key, value)| (key.into(), value.into_property()))
                .collect(),
        ))
    }
}
impl IntoProperty for time::OffsetDateTime {
    fn into_property(self) -> Property {
        let offset = self.offset();
        format!(
            "{}T{:02}:{:02}:{:02}{}{:02}:{:02}",
            self.date(),
            self.hour(),
            self.minute(),
            self.second(),
            if offset.is_negative() { '-' } else { '+' },
            offset.whole_hours().unsigned_abs(),
            offset.minutes_past_hour().unsigned_abs()
        )
        .into_property()
    }
}
impl IntoProperty for time::Date {
    fn into_property(self) -> Property {
        self.to_string().into_property()
    }
}
macro_rules! property_values {
    ($($ty:ty),*) => { $(impl IntoProperty for $ty { fn into_property(self) -> Property { Property(PropertyValue::Json(Value::from(self))) } })* };
}
property_values!(
    Value, &str, String, bool, i8, i16, i32, i64, u8, u16, u32, u64, f32, f64
);
impl IntoProperty for &String {
    fn into_property(self) -> Property {
        self.clone().into_property()
    }
}
