use std::{
    cell::RefCell,
    collections::{BTreeMap, BTreeSet},
    fmt,
    rc::Rc,
};

use serde::{Serialize, Serializer};
use serde_json::{Value, json};

use crate::{CONTEXT, Entity, IntoProperty, JsonLd};

/// A graph or multi-type collection error.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Error {
    /// The type is not in the generated Schema.org vocabulary.
    InvalidType(String),
    /// A node of this type and key already exists.
    AlreadyExists(String, String),
    /// No node of this type and key exists.
    NotFound(String, String),
}
impl fmt::Display for Error {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::InvalidType(name) => write!(f, "Unknown Schema.org type: {name}"),
            Self::AlreadyExists(name, key) => write!(f, "{name} with key {key:?} already exists"),
            Self::NotFound(name, key) => write!(f, "{name} with key {key:?} does not exist"),
        }
    }
}
impl std::error::Error for Error {}

#[derive(Debug)]
struct GraphData {
    nodes: Vec<(String, Entity)>,
    hidden_types: BTreeSet<String>,
    hidden_nodes: BTreeMap<(String, String), bool>,
    context: Value,
    nonce: String,
}

/// A shared JSON-LD graph with nodes keyed by type and a local identifier.
/// Local keys do not become JSON-LD identifiers: use `identifier` on each node for that.
/// Clones share graph membership, visibility, and nodes.
#[derive(Debug, Clone)]
pub struct Graph {
    data: Rc<RefCell<GraphData>>,
}
impl Default for Graph {
    fn default() -> Self {
        Self::new()
    }
}

impl Graph {
    /// Default local key used by generated type accessors.
    pub const IDENTIFIER_DEFAULT: &'static str = "default";

    /// Create an empty graph with the Schema.org context.
    pub fn new() -> Self {
        Self::with_context(CONTEXT)
    }

    /// Create a graph with a custom context (a string, array, or context object).
    pub fn with_context(context: impl IntoProperty) -> Self {
        Self {
            data: Rc::new(RefCell::new(GraphData {
                nodes: Vec::new(),
                hidden_types: BTreeSet::new(),
                hidden_nodes: BTreeMap::new(),
                context: context.into_property().to_value(),
                nonce: String::new(),
            })),
        }
    }

    /// Get the configured context as an owned JSON value.
    pub fn get_context(&self) -> Value {
        self.data.borrow().context.clone()
    }

    /// Get shared handles to all nodes with their local keys, including hidden nodes.
    pub fn get_nodes(&self) -> Vec<(String, Entity)> {
        self.data.borrow().nodes.clone()
    }

    /// Test whether a node exists, including hidden nodes.
    pub fn has(&self, type_name: &str, key: &str) -> bool {
        self.get(type_name, key).is_ok()
    }

    /// Get a shared node handle, including hidden nodes.
    ///
    /// # Errors
    /// Returns [`Error::NotFound`] when the key is absent.
    pub fn get(&self, type_name: &str, key: &str) -> Result<Entity, Error> {
        self.data
            .borrow()
            .nodes
            .iter()
            .find(|(k, node)| k == key && node.get_type() == type_name)
            .map(|(_, node)| node.clone())
            .ok_or_else(|| Error::NotFound(type_name.into(), key.into()))
    }

    /// Insert a shared node, rejecting duplicates.
    ///
    /// # Errors
    /// Returns [`Error::AlreadyExists`] for an occupied type/key pair.
    pub fn add<T>(&mut self, node: &Entity<T>, key: &str) -> Result<&mut Self, Error> {
        if self.has(node.get_type(), key) {
            return Err(Error::AlreadyExists(node.get_type().into(), key.into()));
        }
        Ok(self.set(node, key))
    }

    /// Insert or replace a shared node without changing its position.
    pub fn set<T>(&mut self, node: &Entity<T>, key: &str) -> &mut Self {
        {
            let mut data = self.data.borrow_mut();
            if let Some((_, old)) = data
                .nodes
                .iter_mut()
                .find(|(k, old)| k == key && old.get_type() == node.get_type())
            {
                *old = node.erased();
            } else {
                data.nodes.push((key.into(), node.erased()));
            }
        }
        self
    }

    /// Remove a node and return a shared handle, if present.
    pub fn remove(&mut self, type_name: &str, key: &str) -> Option<Entity> {
        let mut data = self.data.borrow_mut();
        let index = data
            .nodes
            .iter()
            .position(|(k, node)| k == key && node.get_type() == type_name)?;
        Some(data.nodes.remove(index).1)
    }

    /// Get or create a vocabulary type under a local key.
    ///
    /// # Errors
    /// Returns [`Error::InvalidType`] for unknown type names. Use `set` for custom types.
    pub fn get_or_create(&mut self, type_name: &str, key: &str) -> Result<Entity, Error> {
        if !crate::TYPE_NAMES.contains(&type_name) {
            return Err(Error::InvalidType(type_name.into()));
        }
        Ok(self.ensure(type_name, key))
    }

    pub(crate) fn ensure(&mut self, type_name: &str, key: &str) -> Entity {
        let mut data = self.data.borrow_mut();
        if let Some((_, node)) = data
            .nodes
            .iter()
            .find(|(k, node)| k == key && node.get_type() == type_name)
        {
            return node.clone();
        }
        let node = Entity::custom(type_name);
        data.nodes.push((key.into(), node.clone()));
        node
    }

    /// Hide one key, or every node of this type with `None`. Hidden nodes remain accessible.
    pub fn hide(&mut self, type_name: &str, key: Option<&str>) -> &mut Self {
        {
            let mut data = self.data.borrow_mut();
            if let Some(key) = key {
                if !data.hidden_types.contains(type_name) {
                    data.hidden_nodes
                        .insert((type_name.into(), key.into()), true);
                }
            } else {
                data.hidden_types.insert(type_name.into());
                data.hidden_nodes.retain(|(name, _), _| name != type_name);
            }
        }
        self
    }

    /// Show one key, or every node of this type with `None`.
    /// Showing one node after hiding its type keeps the other existing nodes hidden.
    pub fn show(&mut self, type_name: &str, key: Option<&str>) -> &mut Self {
        {
            let mut data = self.data.borrow_mut();
            if let Some(key) = key {
                if data.hidden_types.remove(type_name) {
                    let hidden: Vec<_> = data
                        .nodes
                        .iter()
                        .filter(|(_, node)| node.get_type() == type_name)
                        .map(|(k, _)| ((type_name.into(), k.clone()), k != key))
                        .collect();
                    data.hidden_nodes.extend(hidden);
                }
                data.hidden_nodes
                    .insert((type_name.into(), key.into()), false);
            } else {
                data.hidden_types.remove(type_name);
                data.hidden_nodes.retain(|(name, _), _| name != type_name);
            }
        }
        self
    }

    /// Apply a callback only when the condition is true.
    pub fn when(&mut self, condition: bool, callback: impl FnOnce(&mut Self)) -> &mut Self {
        if condition {
            callback(self);
        }
        self
    }

    /// Set the CSP nonce for script output.
    pub fn set_nonce(&mut self, nonce: impl Into<String>) -> &mut Self {
        self.data.borrow_mut().nonce = nonce.into();
        self
    }
}

impl JsonLd for Graph {
    fn to_value(&self) -> Value {
        let data = self.data.borrow();
        let mut types = Vec::new();
        let mut nodes = Vec::new();
        for (_, node) in &data.nodes {
            if types.contains(&node.get_type()) {
                continue;
            }
            types.push(node.get_type());
            for (key, candidate) in &data.nodes {
                if candidate.get_type() == node.get_type()
                    && !data.hidden_types.contains(node.get_type())
                    && data
                        .hidden_nodes
                        .get(&(node.get_type().into(), key.clone()))
                        != Some(&true)
                {
                    nodes.push(candidate.node());
                }
            }
        }
        json!({"@context": data.context, "@graph": nodes})
    }
    fn nonce(&self) -> String {
        self.data.borrow().nonce.clone()
    }
}

/// One entity described by multiple Schema.org types. Later types win property conflicts.
/// Clones share the component types, including subsequent additions and replacements.
#[derive(Debug, Clone, Default)]
pub struct MultiTypedEntity {
    pub(crate) graph: Graph,
}
impl MultiTypedEntity {
    /// Create an empty multi-typed entity.
    pub fn new() -> Self {
        Self::default()
    }

    /// Add a type, rejecting duplicate types.
    ///
    /// # Errors
    /// Returns [`Error::AlreadyExists`] when the type is already present.
    pub fn add<T>(&mut self, node: &Entity<T>) -> Result<&mut Self, Error> {
        self.graph.add(node, Graph::IDENTIFIER_DEFAULT)?;
        Ok(self)
    }

    /// Insert or replace a shared type.
    pub fn set<T>(&mut self, node: &Entity<T>) -> &mut Self {
        self.graph.set(node, Graph::IDENTIFIER_DEFAULT);
        self
    }

    /// Test whether a type is present.
    pub fn has(&self, type_name: &str) -> bool {
        self.graph.has(type_name, Graph::IDENTIFIER_DEFAULT)
    }

    /// Get a shared component type handle.
    ///
    /// # Errors
    /// Returns [`Error::NotFound`] for absent types.
    pub fn get(&self, type_name: &str) -> Result<Entity, Error> {
        self.graph.get(type_name, Graph::IDENTIFIER_DEFAULT)
    }

    /// Get or create a component type.
    ///
    /// # Errors
    /// Returns [`Error::InvalidType`] for unknown vocabulary types.
    pub fn get_or_create(&mut self, type_name: &str) -> Result<Entity, Error> {
        self.graph
            .get_or_create(type_name, Graph::IDENTIFIER_DEFAULT)
    }

    /// Get shared component type handles in insertion order.
    pub fn get_nodes(&self) -> Vec<Entity> {
        self.graph
            .get_nodes()
            .into_iter()
            .map(|(_, node)| node)
            .collect()
    }

    /// Get the Schema.org context.
    pub fn get_context(&self) -> &str {
        CONTEXT
    }

    /// Apply a callback only when the condition is true.
    pub fn when(&mut self, condition: bool, callback: impl FnOnce(&mut Self)) -> &mut Self {
        if condition {
            callback(self);
        }
        self
    }

    /// Set the CSP nonce for script output.
    pub fn set_nonce(&mut self, nonce: impl Into<String>) -> &mut Self {
        self.graph.set_nonce(nonce);
        self
    }
}
impl JsonLd for MultiTypedEntity {
    fn to_value(&self) -> Value {
        let mut result = json!({"@context": CONTEXT, "@type": []});
        let mut types = Vec::new();
        for node in self.get_nodes() {
            types.push(Value::String(node.get_type().into()));
            if let Value::Object(properties) = node.node() {
                for (key, value) in properties {
                    if key != "@type" && key != "@context" {
                        result[&key] = value;
                    }
                }
            }
        }
        result["@type"] = types.into();
        result
    }
    fn nonce(&self) -> String {
        self.graph.nonce()
    }
}

macro_rules! collection_output {
    ($($ty:ty),*) => {$(
        impl Serialize for $ty {
            fn serialize<S: Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> { self.to_value().serialize(serializer) }
        }
        impl fmt::Display for $ty {
            fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result { f.write_str(&self.to_script()) }
        }
    )*};
}
collection_output!(Graph, MultiTypedEntity);
